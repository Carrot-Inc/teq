object U2:
  def make(): BasicQueue = new BasicQueue with Incrementing2 with Filtering2

trait Incrementing2 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 2)

trait Filtering2 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 2 then super.put(x)
