object U8:
  def make(): BasicQueue = new BasicQueue with Incrementing8 with Filtering8

trait Incrementing8 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 8)

trait Filtering8 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 8 then super.put(x)
