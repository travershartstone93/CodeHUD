#!/bin/bash

# Extract all crate summaries and build prompt
cd "/home/travers/Desktop/CodeHUD (copy)/Rust_copy"

# Build the prompt with all crate summaries
CRATE_SUMMARIES=$(jq -r '.[] | "=== CRATE: " + (.crate_name | ascii_upcase) + " ===\nSummary (" + (.token_count | tostring) + " tokens):\n" + .summary_text + "\n\n"' project_scan_output/crate_summaries.json)

# Create the full prompt
PROMPT="Based on the following crate-level summaries, provide a comprehensive project analysis:

$CRATE_SUMMARIES

Generate a comprehensive technical project summary with TWO sections:

## Overall Architecture
[Describe the system design, architectural patterns, and how components work together as a unified system. List SPECIFIC external libraries/frameworks BY NAME. Explain the layered architecture and data flow between components.]

## What Does It Actually Do
[PRIMARY PURPOSE: Start with the MAIN problem this project solves and its CORE user-facing functionality. What is the #1 thing users can DO with this tool? What OUTPUT does it produce? State the most important capability FIRST, then describe user workflows, use cases, and secondary features. What problem does this solve? How do users interact with it?]

CRITICAL INSTRUCTIONS:
- Identify the PRIMARY purpose first - the main reason this project exists and what users actually DO with it
- Focus on USER-FACING capabilities, not just internal operations
- If this generates summaries/reports/analysis, STATE THAT PROMINENTLY
- Use concrete details from the summaries
- Maximum 500 words total

Focus on the unified purpose and user-facing capabilities, not individual crate descriptions."

# Send to Ollama 14B model
echo "Sending prompt to qwen2.5-coder:14b-instruct-q4_K_M..."
echo "Prompt length: $(echo "$PROMPT" | wc -c) characters"

curl -s http://localhost:11434/api/generate -d "{
  \"model\": \"qwen2.5-coder:14b-instruct-q4_K_M\",
  \"prompt\": $(echo "$PROMPT" | jq -Rs .),
  \"system\": \"You are an expert software architect. Analyze the complete system architecture, component interactions, and unified capabilities. Provide comprehensive, detailed analysis focusing on what users can DO with this system.\",
  \"stream\": false,
  \"options\": {
    \"temperature\": 0.7,
    \"top_p\": 0.9,
    \"top_k\": 40,
    \"num_predict\": 8192,
    \"num_ctx\": 16384
  }
}" | jq -r '.response' > project_scan_output/single_pass_summary.md

echo ""
echo "✅ Single-pass summary generated!"
echo "📄 Saved to: project_scan_output/single_pass_summary.md"
echo ""
echo "Preview:"
head -50 project_scan_output/single_pass_summary.md
