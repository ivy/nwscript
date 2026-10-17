-- Scripts that change the world, from KotOR 1.
--   nws --game k1 -I "$NWSCRIPT_CORPUS" -I "$NWSCRIPT_CORPUS/ncs" test docs/examples

describe("k_pman_planet77: advance the Tortuga plot", function()
  it("adds one to the global", function()
    local w = nws.world()
    w.globals.MAN_TORTB_STATE = 2
    nws.script("k_pman_planet77"):run(w)
    assert.equal(3, w.globals.MAN_TORTB_STATE)
  end)
end)

describe("k_ptat_canhunt: the hunting permit", function()
  it("gives the PC speaker the permit", function()
    local w = nws.world()
    local pc = w:create("creature", "player", { is_pc = true })
    w.speaker = pc
    local run = nws.script("k_ptat_canhunt"):run(w)
    assert.called(run, "CreateItemOnObject", "tat17_huntpermit", pc)
    assert.equal("tat17_huntpermit", pc.inventory[1].tag)
  end)
end)

describe("k_swg_bastila29: a pause in Bastila's conversation", function()
  it("pauses, fades in, waits and resumes, in that order", function()
    local run = nws.script("k_swg_bastila29"):run(nws.world())
    assert.called_before(run, "ActionPauseConversation", "ActionWait")
    assert.called_before(run, "ActionWait", "ActionResumeConversation")
    assert.called(run, "ActionWait", 1.5)
    -- The fade is outside the world model, and the run says so.
    assert.equal("SetGlobalFadeIn", run.unmodelled[1])
  end)
end)
