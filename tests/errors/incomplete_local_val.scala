// A local val whose header ends at a token that cannot continue it is incomplete: its name stands
// with the type it declares, so the use after it resolves, and its missing initializer is not
// reported.
// expect: expected '=', found 'then'
// expect: 1 error found
object O:
  def f: Int =
    val x: Int then
    x
