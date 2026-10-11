// A stand-in for scalajs-env-jsdom-nodejs's class, of its name and shape, which the plugin does not
// depend on: the export reads a jsdom environment by its class's name and its field `config`.
package org.scalajs.jsenv.jsdomnodejs

class JSDOMNodeJSEnv(config: JSDOMNodeJSEnv.Config) {
  override def toString: String = s"JSDOMNodeJSEnv($config)"
}

object JSDOMNodeJSEnv {
  final class Config(val executable: String, val args: List[String], val env: Map[String, String])
}
