object U3:
  def make(): BasicQueue = new BasicQueue with Incrementing3 with Filtering3

trait Incrementing3 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 3)

trait Filtering3 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 3 then super.put(x)
