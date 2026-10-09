// Scala.js's reflective instantiation. The classes and objects that carry
// `@EnableReflectiveInstantiation`, or inherit it, are registered by the compiler with the runtime
// (`$reflectClass`, `$reflectModule` in rt.js) when a lookup is reached, and only then; a lookup
// wraps the registered entry once. `org.portablescala.reflect`, which the `_sjs1_` jars name, is
// the same API.
package scala.scalajs.reflect:

  final class LoadableModuleClass private[reflect] (val runtimeClass: Class[?], loadModuleFun: () => Any):
    def loadModule(): Any = loadModuleFun()

  final class InstantiatableClass private[reflect] (val runtimeClass: Class[?], val declaredConstructors: List[InvokableConstructor]):
    def newInstance(): Any = getConstructor() match
      case Some(ctor) => ctor.newInstance()
      case None =>
        val missing = new NoSuchMethodException(runtimeClass.getName + ".<init>()")
        throw withCause(new InstantiationException(runtimeClass.getName), missing)

    def getConstructor(parameterTypes: Class[?]*): Option[InvokableConstructor] =
      declaredConstructors.find(_.parameterTypes.sameElements(parameterTypes))

  final class InvokableConstructor private[reflect] (val parameterTypes: List[Class[?]], newInstanceFun: Array[Any] => Any):
    def newInstance(args: Any*): Any =
      require(args.size == parameterTypes.size)
      newInstanceFun(args.toArray)

  object Reflect:
    def lookupLoadableModuleClass(fqcn: String): Option[LoadableModuleClass] =
      val entry = registeredModule(fqcn)
      if isUndefined(entry) then None
      else
        if isUndefined(wrapper(entry)) then setWrapper(entry, new LoadableModuleClass(entryClass(entry), moduleLoader(entry)))
        Some(wrapper(entry).asInstanceOf[LoadableModuleClass])

    def lookupInstantiatableClass(fqcn: String): Option[InstantiatableClass] =
      val entry = registeredClass(fqcn)
      if isUndefined(entry) then None
      else
        if isUndefined(wrapper(entry)) then
          val ctors = List.tabulate(ctorCount(entry))(i => new InvokableConstructor(ctorParams(entry, i).toList, ctorFun(entry, i)))
          setWrapper(entry, new InstantiatableClass(entryClass(entry), ctors))
        Some(wrapper(entry).asInstanceOf[InstantiatableClass])

  // The cause of an exception whose class takes none: the instance answers `getCause` itself.
  @js("$withCause($0, $1)")
  private[reflect] def withCause(e: Throwable, cause: Throwable): Throwable
  @js("$reflectedModules.get($0)")
  private[reflect] def registeredModule(fqcn: String): Any
  @js("$reflectedClasses.get($0)")
  private[reflect] def registeredClass(fqcn: String): Any
  @js("($0 === undefined)")
  private[reflect] def isUndefined(x: Any): Boolean
  @js("$0.wrapper")
  private[reflect] def wrapper(entry: Any): Any
  @js("void ($0.wrapper = $1)")
  private[reflect] def setWrapper(entry: Any, w: Any): Unit
  @js("$0.cls")
  private[reflect] def entryClass(entry: Any): Class[?]
  @js("$0.load")
  private[reflect] def moduleLoader(entry: Any): () => Any
  @js("$0.ctors.length")
  private[reflect] def ctorCount(entry: Any): Int
  @js("$0.ctors[$1][0]()")
  private[reflect] def ctorParams(entry: Any, i: Int): Array[Class[?]]
  @js("$0.ctors[$1][1]")
  private[reflect] def ctorFun(entry: Any, i: Int): Array[Any] => Any

package org.portablescala.reflect:

  type LoadableModuleClass = scala.scalajs.reflect.LoadableModuleClass
  type InstantiatableClass = scala.scalajs.reflect.InstantiatableClass
  type InvokableConstructor = scala.scalajs.reflect.InvokableConstructor

  object Reflect:
    def lookupLoadableModuleClass(fqcn: String): Option[LoadableModuleClass] =
      scala.scalajs.reflect.Reflect.lookupLoadableModuleClass(fqcn)
    def lookupLoadableModuleClass(fqcn: String, loader: Any): Option[LoadableModuleClass] =
      scala.scalajs.reflect.Reflect.lookupLoadableModuleClass(fqcn)
    def lookupInstantiatableClass(fqcn: String): Option[InstantiatableClass] =
      scala.scalajs.reflect.Reflect.lookupInstantiatableClass(fqcn)
    def lookupInstantiatableClass(fqcn: String, loader: Any): Option[InstantiatableClass] =
      scala.scalajs.reflect.Reflect.lookupInstantiatableClass(fqcn)
