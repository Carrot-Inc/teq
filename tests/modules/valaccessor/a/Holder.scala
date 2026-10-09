package vaa

// A val of an open class read through its accessor, which a downstream subclass overrides.
open class Holder:
  val v: Int = 1
  def twice: Int = v * 2
