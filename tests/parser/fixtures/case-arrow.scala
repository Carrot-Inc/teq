// A case without its `=>`: the lines after it up to the next case are its body.
object O:
  def f(x: Option[Int]): Int = x match
    case Some(n)
      val m = n + 1
      m
    case None => 0
  val bad: String = 3
