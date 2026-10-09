// The compiler recognises these annotations by name; the classes exist so that the imports resolve and
// JSImport.Default / JSImport.Namespace can be named.
package scala.scalajs.js.annotation

import scala.annotation.StaticAnnotation

class JSImport(module: String, name: String) extends StaticAnnotation

object JSImport:
  final val Default = "default"
  final val Namespace = "*"

class JSName(name: String) extends StaticAnnotation
class JSGlobal(name: String = "") extends StaticAnnotation
class JSGlobalScope extends StaticAnnotation
class JSBracketAccess extends StaticAnnotation
class JSExportTopLevel(name: String) extends StaticAnnotation
class JSExport(name: String = "") extends StaticAnnotation
