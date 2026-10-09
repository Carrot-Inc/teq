object U6:
  def make(): BasicQueue = new BasicQueue with Incrementing6 with Filtering6

trait Incrementing6 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 6)

trait Filtering6 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 6 then super.put(x)
