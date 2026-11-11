// A pattern-bound type parameter in a method's inferred result type: `pm`'s result is `b.m`'s type with `t` bound
// by the pattern (`case b: BaseX[t]`), which scalac 3.8.4 widens when it infers the result, so the program compiles
// and runs. teq types it, but cannot write it to TASTy: "cannot write TASTy for a reference to the type parameter t
// outside its scope in pm" (`teq compiler check --products`, and every build). Found by the controls of the
// type-representation part's observation of the pass on 25e6342fb (x12 p12c); the same on master 8907746df3. Its fix
// is the task list's, not that fold's.
class BaseX[X](x0: X):
  def m: X = x0
class CY[T](f: T => Int) extends BaseX[T => Int](f)
def pm(c: CY[?]) = c match
  case b: BaseX[t] => b.m
@main def run(): Unit =
  pm(CY[Int](x => x + 1)) match
    case _: Function1[?, ?] => println("a function")
