package js

// JavaScript values are typed Any. Nothing has to be converted on the way in or out:
//   - String, Boolean, Int and Double are the JS primitives, Char is a one-character string,
//     Long is a BigInt and Unit is undefined
//   - a function value is a JS function with the same parameters
//   - Array[T] is a JS array; every other collection is an object that JS can iterate
//     (for-of, spread, Array.from) because classes with a foreach method get Symbol.iterator
//   - instances of classes are JS objects: methods are JS methods and vals are plain properties
//     (a val that implements a def of a trait is read through a method of that name)
// Nothing here is checked: a value is whatever the JS side hands over.
//
// Bindings to modules are declared with @jsImport("module", "name") on a top-level def or val
// without a body ("default" is the default export, "*" the namespace object), and
// @jsExport("name") turns a top-level def or val into an export of the compiled ES module.

@js("undefined")
@jvm("aconst_null")
def undefined: Any

// teq itself has no null, JS APIs ask for it all the time.
@js("null")
@jvm("aconst_null")
def nullValue: Any

@js("($0 === undefined)")
@jvm("rt $0:L rtcall isUndefinedValue(Ljava/lang/Object;)Z")
def isUndefined(x: Any): Boolean

@js("($0 === null)")
@jvm("$0:L invokestatic java/util/Objects.isNull(Ljava/lang/Object;)Z")
def isNull(x: Any): Boolean

@js("(typeof $0)")
def typeOf(x: Any): String

@js("globalThis")
def globalThis: Any

@js("globalThis[$0]")
def global(name: String): Any

@js("($0)")
@jvm("$0:L")
def cast[T](x: Any): T

// js.obj("className" -> c, "onClick" -> f), or js.obj(pairs*) for any Seq of pairs.
@js("$jsObj(...$0)")
def obj(fields: (String, Any)*): Any

@js("($0)[$1]")
def get(target: Any, name: String): Any

@js("$jsSet($0, $1, $2)")
def set(target: Any, name: String, value: Any): Unit

// target.method(args...), with target as `this`.
@js("($0)[$1](...$2)")
def call(target: Any, method: String, args: Any*): Any

@js("($0)(...$1)")
def apply(function: Any, args: Any*): Any

@js("new ($0)(...$1)")
def construct(constructor: Any, args: Any*): Any

// js.array(1, 2, 3), or js.array(xs*) for any Seq.
@js("[...$0]")
def array(items: Any*): Any

@js("Array.from($0)")
def arrayFrom[T](iterable: Any): Array[T]

// Works for JS arrays and for everything else JS can iterate.
def toList[T](iterable: Any): List[T] = fromArray(buffered(arrayFrom[T](iterable)))

// A thrown value that is no Throwable (a native TypeError, a string), as `catch` sees it;
// throwing one throws the value itself, as under Scala.js.
final case class JavaScriptException(exception: Any) extends RuntimeException:
  override def getMessage: String = stringOf(exception)

@js("$str($0)")
@jvm("$0:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String;")
def stringOf(x: Any): String

// The forms that raised and intercepted JS errors before `throw` and `try` existed, kept for
// the code that uses them; `throw` and `try` are the way now.
@js("$throw($0)")
def throwError(error: Any): Nothing = throw JavaScriptException(error)

def tryCatch[T](body: () => T)(handler: Any => T): T =
  try body()
  catch
    case JavaScriptException(e) => handler(e)
    case e: Throwable => handler(e)

def tryFinally[T](body: () => T)(finalizer: () => Unit): T =
  try body()
  finally finalizer()
