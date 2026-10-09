// expect: trait T overrides nothing
// expect: class X overrides nothing
// expect: class Y overrides nothing
override trait T
override class X
object M:
  override class Y
