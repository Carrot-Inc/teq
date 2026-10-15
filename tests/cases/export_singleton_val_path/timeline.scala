package eng.anim

opaque type Timeline[A] = List[A]
object Timeline:
  def apply[A](items: List[A]): Timeline[A] = items
  def start: String = "start"
  extension [A](t: Timeline[A]) def size: Int = t.length
