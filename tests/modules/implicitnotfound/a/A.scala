package infa

import scala.annotation.implicitNotFound

@implicitNotFound("No codec for ${A}")
trait Codec[A]

trait Extra

object Codec:
  given Codec[Int] = new Codec[Int] {}

def need[T](using @implicitNotFound("Bring a ${T} codec") c: Codec[T]): Int = 1
def both(using c: Codec[Long] & Extra): Int = 2
