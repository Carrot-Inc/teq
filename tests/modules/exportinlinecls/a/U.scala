package util

class Holder:
  val c: lib.Codecs.type = lib.Codecs
  export c.*
object Utils:
  val h = new Holder
