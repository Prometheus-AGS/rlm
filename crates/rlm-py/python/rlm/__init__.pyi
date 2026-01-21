"""Type stubs for RLM Python bindings."""

from typing import Optional

class RlmConfig:
    """Configuration for RLM execution."""
    
    max_iterations: int
    recursion_depth: int
    model: str
    temperature: float
    
    def __init__(
        self,
        max_iterations: int = 50,
        recursion_depth: int = 1,
        model: str = "gpt-4-turbo",
        temperature: float = 0.0,
    ) -> None: ...

class RlmResponse:
    """Response from RLM execution."""
    
    answer: str
    iterations: int
    recursive_calls: int
    total_tokens: int
    duration_ms: int
    success: bool

def execute(
    query: str,
    context: str,
    config: Optional[RlmConfig] = None,
) -> RlmResponse:
    """
    Execute an RLM request.
    
    Args:
        query: The user query to answer
        context: The long context to process
        config: Optional configuration (uses defaults if None)
    
    Returns:
        RlmResponse with the answer and execution metadata
    
    Example:
        >>> import rlm
        >>> response = rlm.execute(
        ...     query="What is the main topic?",
        ...     context="Long document content...",
        ... )
        >>> print(response.answer)
    """
    ...

def version() -> str:
    """Get the RLM library version."""
    ...
