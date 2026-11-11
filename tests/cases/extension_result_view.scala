object Lib:
  opaque type Tagged[+T, P] = T
  object Tagged:
    def unsafeApply[T, P](t: T): Tagged[T, P] = t
    extension [T, P](r: Tagged[T, P]) def value: T = r

import Lib.*

def scale(amount: BigDecimal, by: Tagged[Int, String]): BigDecimal = amount * by.value
def widen(by: Tagged[Int, String]): Long = by.value

@main def run(): Unit =
  println(scale(BigDecimal(3), Tagged.unsafeApply(4)))
  println(widen(Tagged.unsafeApply(5)))
