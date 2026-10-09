package tpgo
val e = { println("  eager param given object"); 1 }
trait Tag { def x: Int }
given listTag(using n: Int): Tag with
  def x: Int = n
