object U9:
  def make(): BasicQueue = new BasicQueue with Incrementing9 with Filtering9

trait Incrementing9 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 9)

trait Filtering9 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 9 then super.put(x)
