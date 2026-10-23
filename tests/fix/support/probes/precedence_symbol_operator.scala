object O {
  extension (x: Int) def `℘=`(y: Int): Int = x - y
  val result = 1 `℘=` 2 + 3
}
@main def run(): Unit = println(O.result)
