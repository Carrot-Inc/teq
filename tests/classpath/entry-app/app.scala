// An `App` object launched by name with arguments: `App.main` stores them through the setter the object
// implements (`scala$App$$_args_$eq`) and runs the body `delayedInit` queued, where `args` is readable.
package entries

object AppMain extends App:
  delayedInit(println("app: [" + args.mkString(",") + "]"))
