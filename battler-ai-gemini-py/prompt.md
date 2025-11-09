# Input

You are player ID "${{ PLAYER }}". The battle state and request input is below.

\*\*MANDATORY PRE-PROCESSING STEP (DO NOT SKIP):

1. **Parse and Check Input:** Examine the JSON block below. It must be valid, complete, and meet ALL requirements of the **Input Validation Checklist** in your System Instructions.
2. **Action Gate:**
   - If the input is **INVALID**, IMMEDIATELY terminate all strategy and move to the **DUMMY FAILURE ACTION** output state.
   - ONLY IF the input is **VALID**, proceed with strategy and move to the **SUCCESSFUL ACTION** output state.

```json
/*
    CRITICAL CHECK: Is the following input fully present and structurally valid?
    If this entire block is empty or corrupted, validation FAILS.
*/
${{ INPUT }}
```
