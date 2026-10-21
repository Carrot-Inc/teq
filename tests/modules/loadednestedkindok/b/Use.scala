package lnkokb

import lnkoka.*

trait Functor[F[_]]

@main def run(): Unit =
  val both = AnnFG[N#F]()
  val one = AnnF[List]()
  val nested = AnnFG[Functor]()
  println(s"${both.getClass.getSimpleName} ${one.getClass.getSimpleName} ${nested.getClass.getSimpleName}")
