package infa

import scala.annotation.implicitNotFound

@implicitNotFound("No codec for ${A}")
trait Codec[A]

object Codec:
  given Codec[Int] = new Codec[Int] {}
