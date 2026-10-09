object U10:
  def make(): BasicQueue = new BasicQueue with Incrementing10 with Filtering10

trait Incrementing10 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 10)

trait Filtering10 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 10 then super.put(x)
