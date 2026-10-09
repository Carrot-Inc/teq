// teq: --werror
// The space of a type pattern over a type parameter without a tag: the type itself (dotc's
// `Typ(T)`), which covers its sealed bound's children for exhaustivity (`decompose`, Space.scala
// 664) and leaves a wildcard after it reachable, an applied one (`F[Int]` under `F[X] <: A`) by its
// applied bound; an unbounded one or one bounded by a case class covers nothing (the two warnings
// scalac 3.8.4 gives on this file, and none of the others).
sealed trait Entry
case object First extends Entry
case class Other(name: String) extends Entry
sealed trait A
case class B() extends A
case class C() extends A
object Main:
  def a[T <: Entry](v: Entry) = v match { case Other(n) => 1; case _: T @unchecked => 2 }
  def b[T <: Entry](v: Entry) = v match { case _: T @unchecked => 2 }
  def d[T](v: Entry) = v match { case _: T @unchecked => 2 }
  def e[T <: Entry](v: Entry) = v match { case First => 1; case _: T @unchecked => 2 }
  def f[T <: A](v: A) = v match { case _: T @unchecked => 2 }
  def g[T <: A](v: A) = v match { case _: T @unchecked => 2; case _ => 3 }
  def h[T <: B](v: A) = v match { case _: T @unchecked => 2 }
  def i[T <: A](v: A) = v match { case _: T @unchecked => 2; case B() => 3 }
  def j[F[X] <: A](v: A) = v match { case _: F[Int] @unchecked => 2 }
  def k[F[X] <: A](v: A) = v match { case _: F[Int] @unchecked => 2; case _ => 3 }
  def main(args: Array[String]): Unit = ()
// expect: 16:24: warning: match may not be exhaustive; missing: First, Other(_)
// expect: 20:25: warning: match may not be exhaustive; missing: B, C
// expect: 2 warnings found, errors under --werror
