object UseMacroWorkers3:
  def use(x: Any): Boolean = MacroWorkers.isString(x)
