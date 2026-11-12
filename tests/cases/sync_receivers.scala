//> using platform jvm
// `x.synchronized` holds x's monitor for its body alone, whatever the receiver: `this` in a class, an object and a
// trait method, a bare `synchronized`, a class literal, a local, a field, an object named from outside, and an
// expression with an effect, which runs once and before the body (dotty's genSynchronized: genLoadQualifier, then
// monitorenter). Thread.holdsLock answers true inside and false after.
trait Locking:
  def inTrait(): (Boolean, Boolean) =
    val inside = this.synchronized(Thread.holdsLock(this))
    (inside, Thread.holdsLock(this))

class Box extends Locking:
  val field = new Object
  def viaThis(): (Boolean, Boolean) = (this.synchronized(Thread.holdsLock(this)), Thread.holdsLock(this))
  def bare(): (Boolean, Boolean) = (synchronized(Thread.holdsLock(this)), Thread.holdsLock(this))
  def viaField(): (Boolean, Boolean) = (field.synchronized(Thread.holdsLock(field)), Thread.holdsLock(field))

object Single:
  def viaThis(): (Boolean, Boolean) = (this.synchronized(Thread.holdsLock(Single)), Thread.holdsLock(Single))
  def bare(): (Boolean, Boolean) = (synchronized(Thread.holdsLock(this)), Thread.holdsLock(this))

class Marker

object Main:
  var log = List.empty[String]
  def receiver(o: AnyRef): AnyRef =
    log = "receiver" :: log
    o

  def main(args: Array[String]): Unit =
    val b = new Box
    println("class this " + b.viaThis())
    println("class bare " + b.bare())
    println("field " + b.viaField())
    println("trait " + b.inTrait())
    println("object this " + Single.viaThis())
    println("object bare " + Single.bare())
    println("object from outside " + (Single.synchronized(Thread.holdsLock(Single)), Thread.holdsLock(Single)))
    val k = classOf[Marker]
    println("class literal " + (classOf[Marker].synchronized(Thread.holdsLock(k)), Thread.holdsLock(k)))
    val local = new Object
    println("local " + (local.synchronized(Thread.holdsLock(local)), Thread.holdsLock(local)))
    val target = new Object
    val inside = receiver(target).synchronized {
      log = "body" :: log
      Thread.holdsLock(target)
    }
    println("expression " + (inside, Thread.holdsLock(target)) + " " + log.reverse.mkString(","))
