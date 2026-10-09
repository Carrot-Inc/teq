trait Co[+A]
trait Contra[-A]
trait A
trait B

given co: Co[A] & Co[B] = null
given contra: Contra[A] & Contra[B] = null

def useCo[X <: A](using X <:< B, Co[X]): Int = 4
def useContra[X >: A](using B <:< X, Contra[X]): Int = 5

@main def main(): Unit =
  println(useCo)
  println(useContra)
