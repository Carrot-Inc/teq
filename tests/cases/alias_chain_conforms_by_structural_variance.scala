// An application of an alias written without variance signs is compared by the variances its right side gives its
// parameters: `N64[B] <: N64[A]` is `B <: A`, with no unfolding of the sixty-four levels.
trait A
trait B extends A
trait F[+T]
type N2[T] = F[F[T]]
type N4[T] = N2[N2[T]]
type N8[T] = N4[N4[T]]
type N16[T] = N8[N8[T]]
type N32[T] = N16[N16[T]]
type N64[T] = N32[N32[T]]
def widen(x: N64[B]): N64[A] = x

// The same through a higher-kinded parameter's own clause and a wildcard argument.
class Inv[T]
type Through[G[+_], T] = G[T]
type Upper[T] = Inv[? <: T]
def lift(x: Through[N64, B]): Through[N64, A] = x
def upper(x: Upper[N64[B]]): Upper[N64[A]] = x

@main def run(): Unit = println("passed")
