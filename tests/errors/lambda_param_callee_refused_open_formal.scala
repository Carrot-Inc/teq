// Lambdas whose expected type leaves the parameter open and whose body gives dotty no callee to type it
// from (`Typer.typedFunctionValue`'s `calleeType`): the parameter is `Any` (`flipBottom`), and the body
// does not type. A braced block as the body is no application; the parameter inside an argument or under an
// ascription is not the argument (`refersTo`); a parameter twice among the arguments, or not every
// parameter among them, is no index (`paramIndices`); a function part naming the parameter
// (`containsParamRef`); the application is not the body. A by-name parameter of the callee is no type the
// formal is below. A repeated parameter's type is the sequence, which the plain argument then does not fit.
// scalac 3.8.4 refuses each: `Found: (x : Any)` with the callee's parameter type required, `value wrap is not
// a member of Any`, and `Found: (x : Seq[String])  Required: String` for the last.
// expect: 31:28: error: type mismatch: found Any, required String
// expect: 32:29: error: type mismatch: found Any, required String
// expect: 33:28: error: type mismatch: found Any, required String
// expect: 34:25: error: type mismatch: found Any, required String
// expect: 34:28: error: type mismatch: found Any, required String
// expect: 35:33: error: type mismatch: found Any, required String
// expect: 36:21: error: value wrap is not a member of Any
// expect: 37:27: error: type mismatch: found Any, required String
// expect: 38:24: error: type mismatch: found Any, required String
// expect: 39:29: error: type mismatch: found Seq[String], required String
final case class Tag(name: String):
  def wrap(t: Tag): Tag = Tag(name + t.name)
def one(name: String): Tag = Tag(name)
def two(a: String, b: String): Tag = Tag(a + b)
def pair(name: String, n: Int): Tag = Tag(name + n)
def lazily(name: => String): Tag = Tag(name)
def id[T](t: T): T = t
def join(xs: String*): String = xs.mkString("/")
def on[B, R](f: B => R): B => R = f
def on2[B, C, R](f: (B, C) => R): (B, C) => R = f

val braced = on { x => one(x) }
val inside = on(x => one(id(x)))
val ascribed = on(x => one(x: String))
val twice = on(x => two(x, x))
val notAll = on2((s, n) => pair(s, 1))
val named = on(x => x.wrap(x))
val notBody = on(x => one(x).wrap(Tag("c")))
val byName = on(lazily(_))
val repeated = on(x => join(x))
