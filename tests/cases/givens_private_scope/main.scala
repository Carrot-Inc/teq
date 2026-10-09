package shop
package app

import codecs.{Enc, Encs, Sku}
import Encs.given

def encode[T](t: T)(using e: Enc[T]): String = e.enc(t)

@main def run(): Unit =
  println(encode(1))
  println(encode(Sku("a1")))
