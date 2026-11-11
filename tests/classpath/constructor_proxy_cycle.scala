// jars: scala-library fixtures
import fix.shapes.*
object ConstructorProxyCycle {
  val first: CtorCycleLinked = new CtorCycleLinked(1)
  val proxy: CtorCycleLinked = CtorCycleLinked(2)
  val secondary: CtorCycleLinked = CtorCycleLinked()
  val explicitPrimary: CtorCycleLinked = CtorCycleLinked.apply(3)
  val explicitSecondary: CtorCycleLinked = CtorCycleLinked.apply()
  val bounded: CtorCycleBound[CtorCycleBound.Value.type] = CtorCycleBound(CtorCycleBound.Value)
}
