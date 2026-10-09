package ngo
val e = { println("  eager given object"); 1 }
trait Tag { def x: Int }
given instance: Tag with
  def x: Int = 1
