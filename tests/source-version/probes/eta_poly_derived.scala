type P = [A] => A => A
type R[X] = X match { case Int => [A] => A => A }
type B[X] <: [A] => A => A = X match { case Int => [A] => A => A }
trait C { type F <: [A] => A => A; val f: F; val g = f _ }
def t1[F <: [A] => A => A](f: F) = f _
def t2[F <: P](f: F) = f _
def t3(p: P) = p _
def t4(f: ([A] => A => A) | Null) = f _
def t5(f: ([A] => A => A) & Serializable) = f _
def t6(f: R[Int]) = f _
def t7[X](f: R[X]) = f _
def t8[X](f: B[X]) = f _
def t9[G[_] <: [A] => A => A](g: G[Int]) = g _
val pv: P = [A] => (a: A) => a
val q: pv.type = pv
val t10 = q _
