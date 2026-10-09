// One inline method expanded at two sites: the JavaScript output shares one function between the
// expansions (docs/TARGETS.md), also where the expansions waited for the later expansion phase
object M:
  inline def f(x:Int):Int =
    val a=x+1
    val b=a*2
    val c=b+3
    val d=c*4
    d+5
@main def run():Unit =
  println(M.f(1))
  println(M.f(2))
