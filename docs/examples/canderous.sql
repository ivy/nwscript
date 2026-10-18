/* Canderous across the install, by module: every dialogue node he speaks or
   hears (tag Cand), every node whose text names him, every script call that
   passes his party slot (NPC_CANDEROUS, 1) to a party routine, and every
   script and blueprint naming his tag, his blueprint p_cand or his
   conversation k_hcan_dialog. Names and counts only.

   The tree is the one built last; to report on another, replace the `tree`
   CTE's query with `SELECT id FROM trees WHERE label = 'LABEL'`. A module is
   its capsules' stem (STEM.mod, STEM.rim and STEM_s.rim together),
   `Override` for loose files there, and `global` for the BIFs and the rest.
   Counts are of distinct resource names and sites, so the copies of one
   resource in a module's .mod and .rim count once. */
WITH
tree AS (SELECT id FROM trees ORDER BY id DESC LIMIT 1),
loc AS (
  SELECT l.hash, l.name, l.type,
    CASE
      WHEN lower(l.path) LIKE 'modules/%' THEN
        replace(replace(replace(replace(lower(substr(l.path, 9)),
          '_dlg.erf', ''), '_s.rim', ''), '.rim', ''), '.mod', '')
      WHEN lower(l.path) LIKE 'override/%' THEN 'Override'
      ELSE 'global'
    END AS module
  FROM locations l
  WHERE l.tree = (SELECT id FROM tree) AND lower(l.path) NOT LIKE 'lips/%'
),
tlk AS (
  SELECT hash FROM locations
  WHERE tree = (SELECT id FROM tree) AND member = '' AND lower(path) = 'dialog.tlk'
),
-- Nodes he speaks or hears.
speaks AS (
  SELECT DISTINCT loc.module, loc.name || '.' || n.node AS item
  FROM dialog_nodes n JOIN loc ON loc.hash = n.hash
  WHERE n.speaker = 'Cand' COLLATE NOCASE OR n.listener = 'Cand' COLLATE NOCASE
),
-- Nodes whose text names him: embedded text, or the talk table's line.
named AS (
  SELECT DISTINCT loc.module, loc.name || '.' || s.site AS item
  FROM search s JOIN loc ON loc.hash = s.hash
  WHERE search MATCH 'canderous' AND s.kind = 'node'
  UNION
  SELECT DISTINCT loc.module, loc.name || '.' || n.node AS item
  FROM search s
  JOIN dialog_nodes n ON n.strref = CAST(s.site AS INTEGER)
  JOIN loc ON loc.hash = n.hash
  WHERE search MATCH 'canderous' AND s.kind = 'string' AND s.hash IN (SELECT hash FROM tlk)
    AND n.text IS NULL
),
-- Calls passing his party slot to a routine whose first parameter is nNPC.
slot AS (
  SELECT DISTINCT loc.module, loc.name || '@' || c.offset AS item
  FROM calls c JOIN loc ON loc.hash = c.hash
  WHERE c.routine IN (
      'AddAvailableNPCByObject', 'AddAvailableNPCByTemplate', 'AddPartyMember',
      'GetNPCSelectability', 'IsAvailableCreature', 'IsNPCPartyMember',
      'RemoveAvailableNPC', 'RemovePartyMember', 'SetNPCSelectability',
      'SetPartyLeader', 'SpawnAvailableNPC', 'SwitchPlayerCharacter')
    AND json_extract(c.args, '$[0]') = 1
),
-- Scripts and blueprints naming his tag, blueprint or conversation;
-- dialogues' Speaker and Listener are counted above.
named_by AS (
  SELECT DISTINCT loc.module, loc.type, loc.name || ' ' || r.site AS item
  FROM refs r JOIN loc ON loc.hash = r.from_hash
  WHERE ((r.kind = 'tag' AND r.value = 'Cand')
      OR (r.kind = 'resref' AND r.value IN ('p_cand', 'k_hcan_dialog')))
    AND loc.type <> 'dlg'
),
counts AS (
  SELECT module, 'speaks' AS what FROM speaks
  UNION ALL SELECT module, 'named' FROM named
  UNION ALL SELECT module, 'slot' FROM slot
  UNION ALL SELECT module, 'script' FROM named_by WHERE type = 'ncs'
  UNION ALL SELECT module, 'blueprint' FROM named_by WHERE type <> 'ncs'
)
SELECT module,
  sum(what = 'speaks') AS nodes_cand_speaks_or_hears,
  sum(what = 'named') AS nodes_naming_canderous,
  sum(what = 'slot') AS calls_party_slot_1,
  sum(what = 'script') AS script_refs,
  sum(what = 'blueprint') AS gff_refs,
  count(*) AS total
FROM counts
GROUP BY module
ORDER BY module;
