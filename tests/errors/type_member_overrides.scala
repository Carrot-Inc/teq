// scalac checks these after typing, so nothing in this file may fail to type.
// expect: 16:8: error: error overriding type T in trait Bounded with bounds <: String;
// expect: type T, which equals Int has incompatible type
// expect: 22:17: error: error overriding type T in trait Fixed, which equals Int;
// expect: type T, which equals String has incompatible type
// expect: 26:8: error: error overriding type T in trait Narrow with bounds <: Int;
// expect: type T has incompatible type
// expect: 30:8: error: error overriding type T in trait Between with bounds >: Int <: AnyVal;
// expect: type T, which equals String has incompatible type
// expect: 34:8: error: error overriding type G in trait Wrapper with bounds[X] <: Iterable[X];
// expect: type G, which equals Option has incompatible type
// expect: 5 errors found
trait Bounded:
  type T <: String
class B1 extends Bounded:
  type T = Int
trait Fixed:
  type T = Int
class F1 extends Fixed:
  type T = Int
class F2 extends Fixed:
  override type T = String
trait Narrow:
  type T <: Int
class N1 extends Narrow:
  type T
trait Between:
  type T >: Int <: AnyVal
class W1 extends Between:
  type T = String
trait Wrapper:
  type G[X] <: Iterable[X]
class W2 extends Wrapper:
  type G[X] = Option[X]
class W3 extends Wrapper:
  type G[X] = List[X]
trait Fine:
  type T <: Ordered[T]
  type U <: Iterable[U]
@main def run(): Unit = println(B1())
