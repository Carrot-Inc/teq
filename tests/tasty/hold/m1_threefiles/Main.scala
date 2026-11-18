object Main:
  import Far.*
  def main(args: Array[String]): Unit =
    import Near.*
    println(3.m(4))
    println(new Ops("s").m("t"))
