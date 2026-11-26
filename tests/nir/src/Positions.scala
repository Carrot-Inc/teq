def here(): Int = 1

// A program for the NIR writer's fixtures, compiled with the plugin's positionRelativizationPaths naming this
// file itself: its positions are then in the file at the empty path, which Scala Native writes as it writes a
// position in no file, so that a file holds two positions of the same bytes, at its first byte (tests/nir/README).
// No main class: tests/native-link links each one of tests/nir/fixtures against Scala Native's own records.
