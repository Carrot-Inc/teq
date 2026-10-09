package epa

// A file of top-level `export` clauses alone, a prelude its module and the downstream modules
// import: its `Prelude$package` is written as scalac writes it and read back as the package's.
export q.Thing
export q.Tools.{twice, base}
