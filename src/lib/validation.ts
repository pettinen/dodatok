// This file should be automatically generated.
import { format } from "svelte-i18n";

interface Rules {
  minLength?: number;
  maxLength?: number;
}

let _;
format.subscribe(value => { _ = value; });

export const validate = (
  value: string,
  field: string,
  rules: Rules
): string[] => {
  field = _(field);
  const errors = [];
  const length = [...value].length;

  if (rules.minLength && rules.minLength > 0 && length < rules.minLength) {
    if (rules.minLength === 1)
      return [_("validation.empty", { values: { field } })];
    else
      errors.push(_("validation.below-min-length", {
        values: { field, minLength: rules.minLength },
      }));
  }

  if (rules.maxLength && rules.maxLength > 0 && length > rules.maxLength) {
    errors.push(_("validation.above-max-length", {
      values: { field, maxLength: rules.maxLength },
    }));
  }

  return errors;
};

export const passwordRules = {
  minLength: 8,
  maxLength: 1000,
};

export const usernameRules = {
  minLength: 1,
  maxLength: 20,
};
