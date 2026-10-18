//! The tables, their indexes and views. `docs/index.md` describes what each
//! row means; [`VERSION`] changes whenever a table or column does, and a
//! database of another version is rebuilt, never read.

/// The schema's version, kept in `meta` under `schema`.
pub const VERSION: u32 = 4;

/// The tables, created empty.
pub const TABLES: &str = "
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE trees(
  id INTEGER PRIMARY KEY,
  label TEXT NOT NULL UNIQUE,
  parent INTEGER REFERENCES trees(id),
  root TEXT NOT NULL
);
CREATE TABLE resources(
  hash TEXT NOT NULL,
  name TEXT NOT NULL,
  type TEXT NOT NULL,
  size INTEGER NOT NULL,
  first_tree INTEGER NOT NULL REFERENCES trees(id),
  PRIMARY KEY(hash, name, type)
) WITHOUT ROWID;
CREATE TABLE contents(
  hash TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  status TEXT NOT NULL,
  reason TEXT
) WITHOUT ROWID;
CREATE TABLE locations(
  tree INTEGER NOT NULL REFERENCES trees(id),
  path TEXT NOT NULL,
  member TEXT NOT NULL,
  name TEXT NOT NULL,
  type TEXT NOT NULL,
  hash TEXT NOT NULL,
  PRIMARY KEY(tree, path, member)
) WITHOUT ROWID;
CREATE TABLE coverage(
  tree INTEGER NOT NULL REFERENCES trees(id),
  path TEXT NOT NULL,
  member TEXT NOT NULL,
  status TEXT NOT NULL,
  reason TEXT,
  PRIMARY KEY(tree, path, member)
) WITHOUT ROWID;
CREATE TABLE dialog_nodes(
  hash TEXT NOT NULL,
  node TEXT NOT NULL,
  kind TEXT NOT NULL,
  idx INTEGER NOT NULL,
  speaker TEXT,
  listener TEXT,
  strref INTEGER,
  text TEXT,
  voice TEXT,
  sound TEXT,
  script TEXT,
  delay INTEGER,
  camera_angle INTEGER,
  camera_id INTEGER,
  camera_animation INTEGER,
  quest TEXT,
  quest_entry INTEGER,
  comment TEXT,
  animations TEXT
);
CREATE TABLE dialog_links(
  hash TEXT NOT NULL,
  source TEXT NOT NULL,
  position INTEGER NOT NULL,
  target TEXT NOT NULL,
  condition TEXT,
  is_child INTEGER NOT NULL
);
CREATE TABLE gff_fields(
  hash TEXT NOT NULL,
  path TEXT NOT NULL,
  label TEXT NOT NULL,
  type TEXT NOT NULL,
  value,
  strref INTEGER
);
CREATE TABLE cells(
  hash TEXT NOT NULL,
  row INTEGER NOT NULL,
  label TEXT NOT NULL,
  column TEXT NOT NULL COLLATE NOCASE,
  value TEXT NOT NULL
);
CREATE TABLE strings(
  hash TEXT NOT NULL,
  strref INTEGER NOT NULL,
  text TEXT NOT NULL,
  sound TEXT,
  sound_length REAL NOT NULL,
  flags INTEGER NOT NULL
);
CREATE TABLE calls(
  hash TEXT NOT NULL,
  offset INTEGER NOT NULL,
  routine TEXT NOT NULL,
  routine_id INTEGER NOT NULL,
  args TEXT NOT NULL
);
CREATE TABLE resolved_calls(
  hash TEXT NOT NULL,
  site INTEGER NOT NULL,
  offset INTEGER NOT NULL,
  routine TEXT NOT NULL,
  routine_id INTEGER NOT NULL,
  args TEXT NOT NULL,
  path TEXT NOT NULL
);
CREATE TABLE script_strings(
  hash TEXT NOT NULL,
  offset INTEGER NOT NULL,
  value TEXT NOT NULL COLLATE NOCASE
);
CREATE TABLE refs(
  from_hash TEXT NOT NULL,
  kind TEXT NOT NULL,
  value TEXT NOT NULL COLLATE NOCASE,
  site TEXT NOT NULL
);
CREATE VIRTUAL TABLE search USING fts5(
  body,
  kind UNINDEXED,
  hash UNINDEXED,
  site UNINDEXED,
  tokenize = 'unicode61 remove_diacritics 2',
  prefix = '3'
);
";

/// The indexes, created once the rows of a cold build are in, and the
/// views.
pub const INDEXES: &str = "
CREATE INDEX locations_hash ON locations(hash);
CREATE INDEX locations_name ON locations(name, type);
CREATE INDEX resources_name ON resources(name, type);
CREATE INDEX dialog_nodes_hash ON dialog_nodes(hash);
CREATE INDEX dialog_nodes_strref ON dialog_nodes(strref);
CREATE INDEX dialog_links_hash ON dialog_links(hash);
CREATE INDEX gff_fields_hash ON gff_fields(hash);
CREATE INDEX gff_fields_label ON gff_fields(label, value);
CREATE INDEX cells_hash ON cells(hash);
CREATE INDEX strings_hash ON strings(hash, strref);
CREATE INDEX calls_hash ON calls(hash);
CREATE INDEX calls_routine ON calls(routine);
CREATE INDEX resolved_calls_hash ON resolved_calls(hash, offset);
CREATE INDEX resolved_calls_routine ON resolved_calls(routine);
CREATE INDEX script_strings_hash ON script_strings(hash);
CREATE INDEX script_strings_value ON script_strings(value);
CREATE INDEX refs_value ON refs(kind, value);
CREATE INDEX refs_from ON refs(from_hash);
CREATE VIEW tree_resources AS
  SELECT t.label AS tree, l.path, l.member, l.name, l.type, l.hash, c.status
  FROM locations l JOIN trees t ON t.id = l.tree JOIN contents c ON c.hash = l.hash;
CREATE VIEW node_text AS
  SELECT l.tree, l.path, l.member, n.hash, n.node, n.kind, n.idx, n.strref,
         COALESCE(n.text, s.text) AS text
  FROM dialog_nodes n
  JOIN locations l ON l.hash = n.hash
  LEFT JOIN locations k ON k.tree = l.tree AND k.member = '' AND lower(k.path) = 'dialog.tlk'
  LEFT JOIN strings s ON s.hash = k.hash AND s.strref = n.strref;
";

/// The tables whose rows are a content hash's facts, with the column that
/// holds the hash: what is dropped when no location holds the hash any
/// more, and what the digest reads per hash.
pub const FACT_TABLES: &[(&str, &str)] = &[
    ("dialog_nodes", "hash"),
    ("dialog_links", "hash"),
    ("gff_fields", "hash"),
    ("cells", "hash"),
    ("strings", "hash"),
    ("calls", "hash"),
    ("resolved_calls", "hash"),
    ("script_strings", "hash"),
    ("refs", "from_hash"),
    ("search", "hash"),
];
