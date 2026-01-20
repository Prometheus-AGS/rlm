#!/bin/bash
# RLM Server Docker Compose Utilities
#
# This script provides convenient commands for managing RLM server
# deployments using Docker Compose in different environments.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Environment detection
detect_environment() {
    if [ -f "docker-compose.prod.yml" ] && [ "$ENVIRONMENT" = "production" ]; then
        echo "docker-compose.prod.yml"
    elif [ -f "docker-compose.dev.yml" ] && [ "$ENVIRONMENT" = "development" ]; then
        echo "docker-compose.dev.yml"
    else
        echo "docker-compose.yml"
    fi
}

# Function to start services
start_services() {
    local compose_file=$(detect_environment)
    local profile=${PROFILE:-""}

    log_info "Starting RLM server using: $compose_file"

    local cmd="docker-compose -f $compose_file"

    if [ -n "$profile" ]; then
        cmd="$cmd --profile $profile"
    fi

    $cmd up -d

    log_success "Services started successfully"

    # Show status
    show_status
}

# Function to stop services
stop_services() {
    local compose_file=$(detect_environment)

    log_info "Stopping services..."
    docker-compose -f $compose_file down

    log_success "Services stopped"
}

# Function to restart services
restart_services() {
    local compose_file=$(detect_environment)

    log_info "Restarting services..."
    docker-compose -f $compose_file restart

    log_success "Services restarted"
}

# Function to show service status
show_status() {
    local compose_file=$(detect_environment)

    log_info "Service status:"
    docker-compose -f $compose_file ps

    log_info "Health checks:"
    docker-compose -f $compose_file ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"
}

# Function to show logs
show_logs() {
    local compose_file=$(detect_environment)
    local service=${1:-""}
    local follow=${FOLLOW:-false}

    if [ -n "$service" ]; then
        log_info "Showing logs for service: $service"
        if [ "$follow" = "true" ]; then
            docker-compose -f $compose_file logs -f "$service"
        else
            docker-compose -f $compose_file logs --tail=50 "$service"
        fi
    else
        log_info "Showing logs for all services:"
        if [ "$follow" = "true" ]; then
            docker-compose -f $compose_file logs -f
        else
            docker-compose -f $compose_file logs --tail=50
        fi
    fi
}

# Function to execute commands in containers
exec_command() {
    local compose_file=$(detect_environment)
    local service=${1:-"rlm-server"}
    shift

    log_info "Executing command in $service: $*"
    docker-compose -f $compose_file exec "$service" "$@"
}

# Function to scale services
scale_services() {
    local compose_file=$(detect_environment)
    local service=${1:-"rlm-server"}
    local replicas=${2:-2}

    log_info "Scaling $service to $replicas replicas"
    docker-compose -f $compose_file up -d --scale "$service=$replicas"

    log_success "Service scaled successfully"
}

# Function to update services
update_services() {
    local compose_file=$(detect_environment)

    log_info "Updating services..."

    # Pull latest images
    docker-compose -f $compose_file pull

    # Restart with new images
    docker-compose -f $compose_file up -d

    # Clean up old images
    docker image prune -f

    log_success "Services updated successfully"
}

# Function to backup data volumes
backup_volumes() {
    local backup_dir="./backups/$(date +%Y%m%d_%H%M%S)"

    log_info "Creating backup directory: $backup_dir"
    mkdir -p "$backup_dir"

    # Backup volume data
    for volume in $(docker volume ls --format "{{.Name}}" | grep rlm); do
        log_info "Backing up volume: $volume"
        docker run --rm \
            -v "$volume:/source:ro" \
            -v "$(pwd)/$backup_dir:/backup" \
            alpine:latest \
            tar czf "/backup/${volume}.tar.gz" -C /source .
    done

    log_success "Backup completed: $backup_dir"
}

# Function to restore data volumes
restore_volumes() {
    local backup_dir=${1:-""}

    if [ -z "$backup_dir" ] || [ ! -d "$backup_dir" ]; then
        log_error "Backup directory not specified or doesn't exist"
        return 1
    fi

    log_warning "This will overwrite existing volume data. Continue? (y/N)"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        log_info "Restore cancelled"
        return 0
    fi

    for backup_file in "$backup_dir"/*.tar.gz; do
        if [ -f "$backup_file" ]; then
            local volume_name=$(basename "$backup_file" .tar.gz)
            log_info "Restoring volume: $volume_name"

            docker run --rm \
                -v "$volume_name:/target" \
                -v "$(pwd)/$backup_dir:/backup" \
                alpine:latest \
                sh -c "cd /target && tar xzf /backup/$(basename "$backup_file")"
        fi
    done

    log_success "Restore completed"
}

# Function to run health checks
health_check() {
    local compose_file=$(detect_environment)

    log_info "Running health checks..."

    # Check if services are running
    local services=$(docker-compose -f $compose_file ps --services)
    local healthy=true

    for service in $services; do
        local status=$(docker-compose -f $compose_file ps -q "$service" | xargs docker inspect --format='{{.State.Health.Status}}' 2>/dev/null || echo "no-health-check")

        case "$status" in
            "healthy")
                log_success "✓ $service: healthy"
                ;;
            "unhealthy")
                log_error "✗ $service: unhealthy"
                healthy=false
                ;;
            "starting")
                log_warning "⚠ $service: starting"
                ;;
            "no-health-check")
                log_info "? $service: no health check configured"
                ;;
            *)
                log_warning "? $service: unknown status ($status)"
                ;;
        esac
    done

    if [ "$healthy" = true ]; then
        log_success "All services are healthy"
        return 0
    else
        log_error "Some services are unhealthy"
        return 1
    fi
}

# Function to show resource usage
show_resources() {
    local compose_file=$(detect_environment)

    log_info "Resource usage:"

    # Container stats
    docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}" \
        $(docker-compose -f $compose_file ps -q)
}

# Function to show help
show_help() {
    cat << EOF
RLM Server Docker Compose Utilities

Usage: $0 [command] [options]

Commands:
    start              Start all services
    stop               Stop all services
    restart            Restart all services
    status             Show service status
    logs [service]     Show logs (use FOLLOW=true for live logs)
    exec <service> <cmd> Execute command in service container
    scale <service> <n>  Scale service to n replicas
    update             Update services to latest images
    backup             Backup data volumes
    restore <dir>      Restore data volumes from backup
    health             Run health checks
    resources          Show resource usage
    help               Show this help message

Environment Variables:
    ENVIRONMENT        Set to 'development' or 'production' to use specific compose file
    PROFILE           Docker Compose profile to use (e.g., 'monitoring', 'with-cache')
    FOLLOW            Set to 'true' to follow logs in real-time

Examples:
    $0 start
    ENVIRONMENT=production $0 start
    PROFILE=monitoring $0 start
    $0 logs rlm-server
    FOLLOW=true $0 logs
    $0 exec rlm-server bash
    $0 scale rlm-server 3
    $0 backup
    $0 restore ./backups/20240119_143022

EOF
}

# Main script logic
case "${1:-}" in
    "start")
        start_services
        ;;
    "stop")
        stop_services
        ;;
    "restart")
        restart_services
        ;;
    "status")
        show_status
        ;;
    "logs")
        show_logs "$2"
        ;;
    "exec")
        shift
        exec_command "$@"
        ;;
    "scale")
        scale_services "$2" "$3"
        ;;
    "update")
        update_services
        ;;
    "backup")
        backup_volumes
        ;;
    "restore")
        restore_volumes "$2"
        ;;
    "health")
        health_check
        ;;
    "resources")
        show_resources
        ;;
    "help"|"-h"|"--help")
        show_help
        ;;
    "")
        log_info "No command specified. Use 'help' for options."
        show_help
        ;;
    *)
        log_error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac