object Main:
  def visit(stop: Boolean): Unit =
    try
      try
        println("body")
        if stop then return
        println("continue")
      finally println("inner cleanup")
    finally println("outer cleanup")
    println("after both")

  def main(args: Array[String]): Unit =
    visit(true)
    visit(false)
