import model.{Id, Put}

// The companion's given of another file's top-level opaque type, found through the type's implicit scope.
object Repo:
  def find(id: Id): String = summon[Put[Id]].put(id)

@main def run(): Unit = println(Repo.find(Id(7)))
