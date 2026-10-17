-- Dialogue conditions from KotOR 1, tested against the world model.
-- Run with the corpus's scripts on the include path:
--   nws --game k1 -I "$NWSCRIPT_CORPUS" -I "$NWSCRIPT_CORPUS/ncs" test docs/examples

describe("k_con_hk47npm: HK-47 is not in the party", function()
  local script = nws.script("k_con_hk47npm", { game = "k1" })

  it("is true while HK-47 is away", function()
    assert.is_true(script:conditional(nws.world()).result)
  end)

  it("is false once HK-47 joins", function()
    local w = nws.world()
    w.npc_party = { nws.const.NPC_HK_47 }
    local run = script:conditional(w)
    assert.is_false(run.result)
    assert.called(run, "IsNPCPartyMember", nws.const.NPC_HK_47)
  end)
end)

describe("k_pman_misc59: the PC has less than 500 credits", function()
  local script = nws.script("k_pman_misc59")

  it("asks the engine for the PC's gold, which a test supplies", function()
    local w = nws.world()
    local pc = w:create("creature", "player", { is_pc = true })
    w.party = { pc }
    -- GetGold is outside the world model; this test answers it.
    w.engine.GetGold = function(who) return who == pc and 120 or 0 end
    local run = script:conditional(w)
    assert.is_true(run.result)
    assert.modelled(run)
  end)

  it("is false for a rich PC", function()
    local w = nws.world()
    w.party = { w:create("creature", "player", { is_pc = true }) }
    w.engine.GetGold = function() return 9000 end
    assert.is_false(script:conditional(w).result)
  end)
end)

describe("k_pdan_drandom07: a one-in-seven chance", function()
  local script = nws.script("k_pdan_drandom07")

  it("follows the roll", function()
    local w = nws.world()
    w.engine.Random = function(n) assert.equal(7, n) return 0 end
    assert.is_true(script:conditional(w).result)
    w.engine.Random = function() return 4 end
    assert.is_false(script:conditional(w).result)
  end)

  it("lands near one in seven over many seeded worlds", function()
    local hits = 0
    for seed = 1, 700 do
      if script:conditional(nws.world({ seed = seed })).result then hits = hits + 1 end
    end
    assert(hits > 60 and hits < 140, "expected about 100 hits, got " .. hits)
  end)
end)

describe("k_pman_comp35: the Manaan trap is done", function()
  local script = nws.script("k_pman_comp35")

  it("reads the global boolean", function()
    local w = nws.world()
    assert.is_false(script:conditional(w).result)
    w.globals.MAN_PTRAP_DONE = true
    assert.is_true(script:conditional(w).result)
  end)
end)

describe("k_ptar_sithpaper: the PC carries the Sith papers", function()
  local script = nws.script("k_ptar_sithpaper")

  it("is true only with the papers in the PC's inventory", function()
    local w = nws.world()
    local pc = w:create("creature", "player", { is_pc = true })
    w.party = { pc }
    assert.is_false(script:conditional(w).result)
    pc:give("ptar_sithpapers")
    local run = script:conditional(w)
    assert.is_true(run.result)
    assert.called(run, "GetItemPossessedBy", pc, "ptar_sithpapers")
  end)
end)

describe("k_con_canderpm: Canderous is in the party and close by", function()
  local script = nws.script("k_con_canderpm")

  local function world(in_party, distance)
    local w = nws.world()
    local pc = w:create("creature", "player", { is_pc = true, area = w.module, position = { x = 0, y = 0, z = 0 } })
    w:create("creature", "cand", { area = w.module, position = { x = distance, y = 0, z = 0 } })
    w.speaker = pc
    if in_party then w.npc_party = { nws.const.NPC_CANDEROUS } end
    return w
  end

  it("is true when both hold", function()
    assert.is_true(script:conditional(world(true, 4.0)).result)
  end)

  it("is false when he is too far away", function()
    assert.is_false(script:conditional(world(true, 25.0)).result)
  end)

  it("does not measure the distance when he is not in the party: && short-circuits", function()
    local run = script:conditional(world(false, 4.0))
    assert.is_false(run.result)
    assert.not_called(run, "GetDistanceBetween")
  end)
end)

describe("k_pdan_mand04: Jon's state is 0 or 1", function()
  local script = nws.script("k_pdan_mand04")

  it("accepts the first two states only", function()
    for state, want in pairs({ [0] = true, [1] = true, [2] = false, [5] = false }) do
      local w = nws.world()
      w.globals.DAN_JON_STATE = state
      assert.equal(want, script:conditional(w).result, "state " .. state)
    end
  end)
end)
