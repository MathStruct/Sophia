-- Fixture for elab_costs.lean: two consecutive failing commands must each
-- report one error (the message log is reset per command).
theorem bad1 : 1 = 2 := rfl
theorem bad2 : 1 = 2 := rfl
def ok : Nat := 0
