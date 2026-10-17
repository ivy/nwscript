---@meta
-- Type annotations for the `nws` Lua API, for lua-language-server.
-- Point your editor's Lua.workspace.library at this directory.
-- The reference is crates/nwscript-lua/testing.md, printed by `nws test --reference`.

---@class nws
---@field game string the game profile, "k1" or "k2"
---@field any table matches any argument in assert.called
---@field const table<string, integer|number|string> nwscript.nss's constants by name
---@field args string[] arguments after the script's name, under `nws lua`
nws = {}

---Load a compiled script by name: NAME.ncs from the test's directory or an -I directory.
---@param name string
---@param opts? { game?: string }
---@return nws.Script
function nws.script(name, opts) end

---A new, empty world: a module object, no area, nobody in the party.
---@param opts? { seed?: integer }
---@return nws.World
function nws.world(opts) end

---@class nws.Script
---@field name string
local Script = {}

---Run the script's main body, then its deferred actions.
---@param world? nws.World a fresh world when omitted
---@return nws.Run
function Script:run(world) end

---Run a StartingConditional; the run's result is a boolean.
---@param world? nws.World
---@return nws.Run
function Script:conditional(world) end

---@class nws.Run
---@field result any the entry's return value; a boolean for :conditional
---@field fault string? why the run stopped early
---@field unmodelled string[] routines answered by defaults, not a model
---@field trace nws.Call[] every engine call, in order
local Run = {}

---The calls to one routine.
---@param name string
---@return nws.Call[]
function Run:calls(name) end

---The trace as text.
---@return string
function Run:text() end

---@class nws.Call
---@field name string
---@field routine integer
---@field args any[]
---@field ret any
---@field modelled boolean
---@field deferred integer? the deferred action the call ran in

---@class nws.World
---@field globals table<string, integer|boolean|string> global variables by name
---@field engine table<string, fun(...): any> routines this world answers with Lua functions
---@field module nws.Object
---@field self nws.Object OBJECT_SELF
---@field speaker nws.Object GetPCSpeaker
---@field entering nws.Object GetEnteringObject
---@field exiting nws.Object GetExitingObject
---@field opened_by nws.Object GetLastOpenedBy
---@field used_by nws.Object GetLastUsedBy
---@field clicking nws.Object GetClickingObject
---@field perceived nws.Object GetLastPerceived
---@field user_event integer GetUserDefinedEventNumber
---@field party nws.Object[] the party, leader first
---@field npc_party integer[] NPC_* numbers in the party
local World = {}

---Create an object.
---@param kind "creature"|"item"|"trigger"|"door"|"waypoint"|"placeable"|"store"|"area"|integer
---@param tag string
---@param fields? table field values, as on nws.Object
---@return nws.Object
function World:create(kind, tag, fields) end

---The first object with the tag, ignoring case.
---@param tag string
---@return nws.Object?
function World:object(tag) end

---Make two STANDARD_FACTION_* numbers hostile to each other.
---@param a integer
---@param b integer
function World:enemies(a, b) end

---@class nws.Object
---@field id integer
---@field tag string
---@field name string
---@field resref string
---@field hp integer
---@field max_hp integer
---@field dead boolean
---@field race integer
---@field gender integer
---@field faction integer
---@field ai_style integer
---@field plot boolean
---@field in_combat boolean
---@field is_pc boolean
---@field area nws.Object
---@field position { x: number, y: number, z: number }
---@field facing number
---@field base_item integer
---@field destroyed boolean
---@field inventory nws.Object[]
local Object = {}

---Put a new item with this tag in the object's inventory.
---@param tag string
---@return nws.Object
function Object:give(tag) end

---Equip a new item with this tag in an INVENTORY_SLOT_*.
---@param slot integer
---@param tag string
---@return nws.Object
function Object:equip(slot, tag) end

---The creature knows this spell or Force power.
---@param spell integer
function Object:learn(spell) end

---The creature has this feat.
---@param feat integer
function Object:feat(feat) end

---The creature has this many levels in a CLASS_TYPE_*; GetHitDice is their sum.
---@param class integer
---@param levels integer
function Object:level(class, levels) end

---The object is under this spell's effect.
---@param spell integer
function Object:affect(spell) end

---Add a talent: its category, TALENT_TYPE_* and id.
---@param category integer
---@param kind integer
---@param id integer
function Object:talent(category, kind, id) end

---Set a local variable: a boolean or an integer at the index.
---@param index integer
---@param value boolean|integer
function Object:set_local(index, value) end

---Whether the object exists and has not been destroyed.
---@return boolean
function Object:valid() end

---@param name string
---@param body fun()
function describe(name, body) end

---@param name string
---@param body fun()
function it(name, body) end

---@param body fun()
function before_each(body) end

---@class assert
assert = {}
---@param v any
---@param message? string
function assert.is_true(v, message) end
---@param v any
---@param message? string
function assert.is_false(v, message) end
---@param v any
function assert.is_nil(v) end
---@param want any
---@param got any
---@param message? string
function assert.equal(want, got, message) end
---A call to the routine with these leading arguments; nws.any matches any one.
---@param run nws.Run
---@param name string
---@param ... any
function assert.called(run, name, ...) end
---No call to the routine with these leading arguments.
---@param run nws.Run
---@param name string
---@param ... any
function assert.not_called(run, name, ...) end
---The first call to `first` comes before the first call to `second`.
---@param run nws.Run
---@param first string
---@param second string
function assert.called_before(run, first, second) end
---No reply in the run came from defaults.
---@param run nws.Run
function assert.modelled(run) end
