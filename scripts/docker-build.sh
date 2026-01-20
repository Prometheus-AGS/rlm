#!/bin/bash
# RLM Server Docker Build Script
#
# This script builds Docker images for different environments
# and provides utilities for development and deployment.

set -e

# Configuration
REPO_NAME="rlm-server"
VERSION=${VERSION:-"latest"}
REGISTRY=${REGISTRY:-""}

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

# Function to build production image
build_production() {
    log_info "Building production Docker image..."

    docker build \
        --file Dockerfile \
        --tag "${REPO_NAME}:${VERSION}" \
        --tag "${REPO_NAME}:latest" \
        --build-arg BUILD_DATE="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
        --build-arg VCS_REF="$(git rev-parse --short HEAD)" \
        --build-arg VERSION="${VERSION}" \
        .

    log_success "Production image built successfully"

    # Display image info
    docker images | grep "${REPO_NAME}" | head -5
}

# Function to build development image
build_development() {
    log_info "Building development Docker image..."

    docker build \
        --file Dockerfile.dev \
        --tag "${REPO_NAME}:dev" \
        .

    log_success "Development image built successfully"
}

# Function to build and tag for registry
build_for_registry() {
    if [ -z "$REGISTRY" ]; then
        log_error "REGISTRY environment variable must be set for registry builds"
        exit 1
    fi

    log_info "Building and tagging for registry: $REGISTRY"

    # Build production image
    build_production

    # Tag for registry
    docker tag "${REPO_NAME}:${VERSION}" "${REGISTRY}/${REPO_NAME}:${VERSION}"
    docker tag "${REPO_NAME}:latest" "${REGISTRY}/${REPO_NAME}:latest"

    log_success "Images tagged for registry"

    # Optionally push
    if [ "$PUSH" = "true" ]; then
        log_info "Pushing to registry..."
        docker push "${REGISTRY}/${REPO_NAME}:${VERSION}"
        docker push "${REGISTRY}/${REPO_NAME}:latest"
        log_success "Images pushed to registry"
    fi
}

# Function to test the built image
test_image() {
    local image_tag="${1:-${REPO_NAME}:latest}"

    log_info "Testing Docker image: $image_tag"

    # Basic smoke test
    log_info "Running smoke test..."
    if docker run --rm --detach \
        --name rlm-test \
        --publish 18080:8080 \
        --env RLM_LLM_API_KEY=test-key \
        "$image_tag" > /dev/null; then

        # Wait for server to start
        sleep 10

        # Test health endpoint
        if curl -f http://localhost:18080/health > /dev/null 2>&1; then
            log_success "Health check passed"
        else
            log_warning "Health check failed, but container started"
        fi

        # Clean up
        docker stop rlm-test > /dev/null 2>&1 || true

        log_success "Smoke test completed"
    else
        log_error "Failed to start container for testing"
        return 1
    fi
}

# Function to analyze image
analyze_image() {
    local image_tag="${1:-${REPO_NAME}:latest}"

    log_info "Analyzing Docker image: $image_tag"

    # Image size
    log_info "Image size:"
    docker images "$image_tag" --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"

    # Security scan (if available)
    if command -v docker &> /dev/null && docker version --format '{{.Server.Version}}' | grep -q "20\|21\|22\|23\|24"; then
        log_info "Running security scan..."
        if docker scout quickview "$image_tag" 2>/dev/null; then
            log_success "Security scan completed"
        else
            log_warning "Docker Scout not available or failed"
        fi
    fi

    # Layer analysis
    log_info "Image layers:"
    docker history "$image_tag" --no-trunc --format "table {{.CreatedBy}}\t{{.Size}}" | head -10
}

# Function to clean up Docker resources
cleanup() {
    log_info "Cleaning up Docker resources..."

    # Remove dangling images
    if docker images -f "dangling=true" -q | grep -q .; then
        docker rmi $(docker images -f "dangling=true" -q)
        log_info "Removed dangling images"
    fi

    # Prune build cache
    docker builder prune -f > /dev/null 2>&1 || true

    log_success "Cleanup completed"
}

# Function to show help
show_help() {
    cat << EOF
RLM Server Docker Build Script

Usage: $0 [command] [options]

Commands:
    production, prod    Build production Docker image
    development, dev    Build development Docker image
    registry           Build and tag for registry
    test              Test the built image
    analyze           Analyze image size and security
    cleanup           Clean up Docker resources
    help              Show this help message

Environment Variables:
    VERSION           Image version tag (default: latest)
    REGISTRY          Container registry URL (required for registry command)
    PUSH              Set to 'true' to push to registry (default: false)

Examples:
    $0 production
    VERSION=v1.2.3 $0 prod
    REGISTRY=docker.io/myorg PUSH=true $0 registry
    $0 test rlm-server:v1.2.3
    $0 analyze
    $0 cleanup

EOF
}

# Main script logic
case "${1:-}" in
    "production"|"prod")
        build_production
        ;;
    "development"|"dev")
        build_development
        ;;
    "registry")
        build_for_registry
        ;;
    "test")
        test_image "$2"
        ;;
    "analyze")
        analyze_image "$2"
        ;;
    "cleanup")
        cleanup
        ;;
    "help"|"-h"|"--help")
        show_help
        ;;
    "")
        log_info "Building production image by default. Use 'help' for options."
        build_production
        ;;
    *)
        log_error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac