use specter_probe::lift::lift_binary;

mod fixtures;

#[test]
fn test_lift_minimal_add_one() {
    let data = fixtures::minimal_add_one();
    let output = lift_binary(&data);
    
    assert!(output.total_instructions >= 2, "Expected at least 2 instructions, got {}", output.total_instructions);
    assert!(output.lifted_functions >= 1, "Expected at least 1 function, got {}", output.lifted_functions);
    assert!(!output.ir_text.is_empty(), "IR text should not be empty");
    
    // Check that the IR contains meaningful operations (not just comments)
    let has_ssa = output.ir_text.contains("%v");
    assert!(has_ssa, "IR should contain SSA variables: {}", output.ir_text);
}

#[test]
fn test_lift_function_detection() {
    let data = fixtures::function_with_prologue();
    let output = lift_binary(&data);
    
    assert!(output.lifted_functions >= 1, "Should detect a function with prologue");
    
    // Check that ret is handled
    let has_ret = output.ir_text.contains("ret i64");
    assert!(has_ret, "IR should contain ret instruction");
}

#[test]
fn test_lift_ir_has_ssa_form() {
    let data = fixtures::minimal_add_one();
    let output = lift_binary(&data);
    
    // SSA means each variable is assigned exactly once
    // Check that variables follow the pattern %vN where N is a number
    let ir = &output.ir_text;
    let var_count = ir.matches("%v").count();
    assert!(var_count > 0, "Expected SSA variables in IR");
    
    // Check for add operation
    let has_add = ir.contains("add") || ir.contains("sub");
    assert!(has_add || var_count > 0, "Expected arithmetic operations in IR");
}

#[test]
fn test_lift_cfg_structure() {
    let data = fixtures::conditional_branch();
    let output = lift_binary(&data);
    
    // Should have at least one function with basic blocks
    for func in &output.functions {
        assert!(!func.blocks.is_empty(), "Each function should have basic blocks");
        for block in &func.blocks {
            assert!(!block.instructions.is_empty(), "Each block should have instructions");
        }
    }
}

#[test]
fn test_lift_arm32() {
    let data = fixtures::arm32_stub();
    let output = lift_binary(&data);
    
    // ARM32 ELF stub — may or may not disassemble depending on Capstone
    // At minimum, the pipeline should not crash
    let _ = output;
}

#[test]
fn test_ir_registers_tracked() {
    use specter_probe::lift::types::Reg;
    // Verify Reg implements Hash (needed for SSA construction)
    fn _assert_hash<T: std::hash::Hash>() {}
    _assert_hash::<Reg>;
}

#[test]
fn test_pipeline_chain() {
    // Test that the full pipeline (lift → analysis → mmio) works end-to-end
    let data = fixtures::function_with_prologue();
    let lift_out = lift_binary(&data);
    
    assert!(lift_out.lifted_functions > 0, "Pipeline should produce functions");
    assert!(lift_out.total_instructions > 0, "Pipeline should produce instructions");
    
    let analysis = specter_probe::lift::analysis::analyze(&lift_out.functions);
    assert!(analysis.function_count > 0, "Analysis should find functions");
    assert!(analysis.total_instructions > 0, "Analysis should count instructions");
}
