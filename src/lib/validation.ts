// This file should be automatically generated.
import { format } from "svelte-i18n";
import type { MessageFormatter } from "svelte-i18n/types/runtime/types";

import type { ErrorMessage } from "$lib/types";

interface Rules {
  minLength?: number;
  maxLength?: number;
}

let _: MessageFormatter;
format.subscribe((value): void => {
  _ = value;
});

export const validate = (
  source: string,
  fieldName: string,
  value: string,
  rules: Readonly<Rules>
): ErrorMessage[] => {
  const field = _(fieldName);
  const errors = [];
  const { length } = [...value];

  if (
    rules.minLength !== undefined
    && rules.minLength > 0
    && length < rules.minLength
  ) {
    if (rules.minLength === 1) {
      return [
        {
          source: fieldName,
          text: _("validation.empty", { values: { field } }),
        },
      ];
    }
    errors.push(_("validation.below-min-length", {
      values: { field, minLength: rules.minLength },
    }));
  }

  if (
    rules.maxLength !== undefined
    && rules.maxLength > 0
    && length > rules.maxLength
  ) {
    errors.push(_("validation.above-max-length", {
      values: { field, maxLength: rules.maxLength },
    }));
  }

  return errors.map((text): ErrorMessage => ({ source, text }));
};

export const passwordRules = {
  maxLength: 1000,
  minLength: 1,  // DEBUG
};

export const usernameRules = {
  maxLength: 20,
  minLength: 1,
};
