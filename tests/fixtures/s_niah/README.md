# S-NIAH (Simple Needle-in-Haystack) Test Fixtures

This directory contains test fixtures for the S-NIAH evaluation dataset from the MIT RLM paper (arXiv:2512.24601).

## Dataset Overview

S-NIAH tests the ability of language models to find specific information ("needles") within very long documents ("haystacks"). This is a fundamental capability for long-context understanding.

### Complexity: O(1)
The RLM approach should achieve O(1) complexity for needle-in-haystack tasks by:
1. Offloading the full context to REPL
2. Using focused LLM queries to search for specific information
3. Avoiding the need to process the entire context in attention

## Test Structure

Each test case consists of:
- `haystack.txt` - The long document containing mostly irrelevant information
- `needle.txt` - The specific information to be found
- `question.txt` - The question asking about the needle
- `expected.txt` - The expected answer
- `metadata.json` - Test case metadata (length, position, etc.)

## Expected Results

From the paper's evaluation (Table 1):
- **Baseline GPT-4**: 95.0% accuracy
- **RLM with GPT-4**: 97.5% accuracy
- **Performance improvement**: +2.5% accuracy with comparable or lower cost

## Test Cases

### Basic Tests
- `test_001/` - Short needle in medium haystack (10K tokens)
- `test_002/` - Specific fact in long document (50K tokens)
- `test_003/` - Number/date in very long haystack (100K+ tokens)

### Advanced Tests
- `test_004/` - Multiple needles requiring selection
- `test_005/` - Needle at beginning of document
- `test_006/` - Needle at end of document
- `test_007/` - Needle in middle of document

## Usage

These fixtures are used by:
- Golden tests in `tests/golden/test_s_niah.rs`
- Performance benchmarks comparing RLM vs baseline
- Regression testing to ensure accuracy improvements

## Future Additions

TODO: Add actual test cases from MIT RLM paper dataset once available
TODO: Add synthetic test generation scripts
TODO: Add multi-language test cases
TODO: Add tests with different document structures (code, tables, etc.)