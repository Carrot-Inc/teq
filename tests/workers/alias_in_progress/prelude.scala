// A file with quoted code, typed before the fork: what the bodies of main.scala need, the trait
// `Mirror` complete and the signatures they publish, but for the member they race on.
import scala.deriving.Mirror
import scala.quoted.*

def bound[T <: Tuple]: Int = 1

def label(m: Mirror.Of[Int]): Option[m.MirroredLabel] = None

def twiceCode(x: Expr[Int])(using Quotes): Expr[Int] = '{ $x + $x + bound[EmptyTuple] }

def all: Int = A0.f(null) + A1.f(null) + A2.f(null) + A3.f(null) + A4.f(null) + A5.f(null) + A6.f(null) + A7.f(null) + A8.f(null) + A9.f(null) + A10.f(null) + A11.f(null) + A12.f(null) + A13.f(null) + A14.f(null) + A15.f(null)
