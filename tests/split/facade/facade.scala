package fac

object Facade:
  export prov.Provider.{greet, twice}
  def own: String = "facade"
