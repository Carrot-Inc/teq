// lila: Form3.nativeCheckbox[Value: Show](fieldId, fieldName, checked, value: Value = "true")
// called without `value`. The default getter's result type is inferred from its right-hand
// side ("true": String; Namer.expectedDefaultArgType wildcards the method's type params),
// so the omitted argument constrains Value := String and Show[String] is found.
trait Show[A] { def show(a: A): String }
given Show[String] = s => s"'$s'"

def checkbox[Value: Show](id: String, checked: Boolean, value: Value = "true"): String =
  s"$id checked=$checked value=${summon[Show[Value]].show(value)}"

@main def run() =
  println(checkbox("remember", checked = false))
  println(checkbox("agree", checked = true))
