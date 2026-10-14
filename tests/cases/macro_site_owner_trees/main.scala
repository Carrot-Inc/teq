class Coll

class Env:
  val api =
    def mk = (coll: Coll) => inspect
    mk(new Coll)
  val owners =
    def mk = (coll: Coll, n: Int) => params
    mk(new Coll, 1)
  def viaFor = for c <- List(new Coll) yield params

@main def run(): Unit =
  val env = new Env
  println(env.api)
  println(env.owners)
  println(env.viaFor)
