package exc

// An export of an export: the forwarder of C calls B's, which calls A's.
object C:
  export B.{n => k, U => W}
