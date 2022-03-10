import { config } from "$lib/config";
import type { Message } from "$lib/errors";
import type { MaybePromise } from "$lib/types";


const rules = config.validationRules;

export const validators = {
  locale: (value: string): Message[] => {
    if (!(value in config.locales))
      return ["validation.errors.locale.invalid"];
    return [];
  },
  password: (value: string, fieldName: string, enforceMinLength: boolean): Message[] => {
    const { length } = [...value];
    if (length === 0)
      return [`validation.errors.${fieldName}.empty`];
    if (enforceMinLength && length < rules.password.minLength) {
      return [
        [`validation.errors.${fieldName}.too-short`, { minLength: rules.password.minLength }],
      ];
    }
    if (length > rules.password.maxLength)
      return [[`validation.errors.${fieldName}.too-long`, { maxLength: rules.password.maxLength }]];
    return [];
  },
  totp: (value: string, allowEmpty: boolean): Message[] => {
    if (!value)
      return allowEmpty ? [] : ["validation.errors.totp.empty"];

    if (!new RegExp(`[0-9]{${config.totp.codeLength}}$`, "u").test(value))
      return [["validation.errors.totp.invalid", { codeLength: config.totp.codeLength }]];
    return [];
  },
  userIcon: (file: File): MaybePromise<Message[]> => {
    if (!config.acceptedImageTypes.includes(file.type))
      return ["validation.errors.user-icon.invalid-file-type"];
    if (file.size > rules.userIcon.maxSizeMB * 1_000_000) {
      return [
        ["validation.errors.user-icon.too-large", { maxSizeMB: rules.userIcon.maxSizeMB }],
      ];
    }
    return new Promise((resolve) => {
      const reader = new FileReader();
      reader.addEventListener("load", () => {
        const image = new Image();
        image.addEventListener("load", () => {
          if (image.width < rules.userIcon.minSize || image.height < rules.userIcon.minSize) {
            resolve([
              [
                "validation.errors.user-icon.too-small",
                { minDimension: rules.userIcon.minSize },
              ],
            ]);
            return;
          }

          const ratio = image.width / image.height;
          if (ratio > rules.userIcon.maxDimensionRatio)
            resolve(["validation.errors.user-icon.too-wide"]);
          else if (ratio < 1 / rules.userIcon.maxDimensionRatio)
            resolve(["validation.errors.user-icon.too-tall"]);
          else
            resolve([]);
        });
        image.addEventListener("error", () => {
          resolve(["validation.errors.user-icon.invalid-image"]);
        });
        if (typeof reader.result === "string")
          image.src = reader.result;
      });
      reader.addEventListener("error", () => {
        resolve(["validation.errors.user-icon.invalid-image"]);
      });
      reader.readAsDataURL(file);
    });
  },
  username: (value: string, fieldName: string, enforceMinLength: boolean): Message[] => {
    const { length } = [...value];
    if (length === 0)
      return [`validation.errors.${fieldName}.empty`];
    if (enforceMinLength && length < rules.username.minLength) {
      return [
        [`validation.errors.${fieldName}.too-short`, { minLength: rules.username.minLength }],
      ];
    }
    if (length > rules.username.maxLength)
      return [[`validation.errors.${fieldName}.too-long`, { maxLength: rules.username.maxLength }]];
    return [];
  },
};
