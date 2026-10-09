// A function literal without parameters passed to an overloaded method or constructor whose
// alternatives differ in a trait parameter: its shape picks the alternative, which takes it
// through SAM conversion (scala3's pos/i2367, `new Thread(() => ...)`).
trait Job:
  def run(): Unit

object W:
  def go(name: String, job: Job): Unit = job.run()
  def go(job: Job): Unit = job.run()
  def go(name: String): Unit = println(name)

class Worker(name: String, job: Job):
  def this(job: Job) = this("w", job)
  def this(name: String) = this(name, null)
  def go(): Unit = job.run()

object Main:
  def main(args: Array[String]): Unit =
    W.go(() => println("method"))
    W.go("named", () => println("two"))
    new Worker(() => println("constructor")).go()
    new Worker("n", () => println("primary")).go()
    val r: Runnable = () => println("runnable")
    r.run()
