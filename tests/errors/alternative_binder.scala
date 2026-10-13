// A variable bound inside a pattern alternative, directly or nested in a constructor pattern, is scalac's
// E024 at every binder (dotty's `typedAlternative` sets `Mode.InPatternAlternative`, which `typedBind`
// refuses); a binder around the alternative and wildcards inside it are not. Master accepted both and
// failed at run time.
// expect: 10:9: error: Illegal variable a in pattern alternative
// expect: 10:20: error: Illegal variable a in pattern alternative
// expect: 14:13: error: Illegal variable a in pattern alternative
// expect: 14:23: error: Illegal variable a in pattern alternative
def f(x: Any): Any = x match
  case (a: Int) | (a: String) => a
  case _ => 0

def g(x: Any): Any = x match
  case Some(a) | List(a) => a
  case _ => 0

def h(x: Any): Any = x match
  case y @ (_: Int | _: String) => y
  case Some(_) | List(_) => 1
  case _ => 0
