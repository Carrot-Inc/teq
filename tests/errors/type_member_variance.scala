// expect: 10:8: error: covariant type X occurs in contravariant position in type  >: X of type Bad
// expect: 12:8: error: contravariant type X occurs in covariant position in type  <: X of type Bad
// expect: 14:8: error: covariant type X occurs in invariant position in type  = List[X] of type Alias
// expect: 3 errors found
trait Wrapper:
  type G[X] <: Iterable[X]
class W3 extends Wrapper:
  type G[X] = List[X]
class Box[+X]:
  type Bad >: X
class Box2[-X]:
  type Bad <: X
class Box3[+X]:
  type Alias = List[X]
  type Good <: X
@main def run(): Unit = println(W3())
