abstract class Queue:
  def put(x: Int): Unit

class BasicQueue extends Queue:
  var items: List[Int] = Nil
  def put(x: Int): Unit = items = items :+ x
