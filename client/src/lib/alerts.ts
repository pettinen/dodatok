import { deepEqual } from "fast-equals";
import { derived, writable } from "svelte/store";
import type { Readable, Writable } from "svelte/store";

import type { Alert as ApiAlert, AlertParams } from "helpers";

import { tl } from "$i18n";

type AlertType = "error" | "warning";

export class Alert {
    public readonly message_id: string;
    public readonly type: AlertType;
    public readonly params: AlertParams | null;

    public constructor(
        source: string,
        type: AlertType,
        id: string,
        params?: AlertParams,
    ) {
        this.message_id = `${source}.${type}s.${id}`;
        this.type = type;
        this.params = params ?? null;
    }

    public get tl(): [string, AlertParams | null] {
        return [this.message_id, this.params];
    }

    public static from_api_alert(
        type: AlertType,
    ): (api_alert: ApiAlert) => Alert {
        return (api_alert) => {
            const { source, id, params } = api_alert;
            return new Alert(source, type, id, params);
        };
    }

    public static from_api_warning(api_alert: ApiAlert): Alert {
        const { source, id, params } = api_alert;
        return new Alert(source, "warning", id, params);
    }
}

interface AlertStore extends Writable<Alert[]> {
    add_from_api: (...alerts: ApiAlert[]) => void;
    clear: (...searches: Array<Alert | RegExp | string>) => void;
}

const alert_filter =
    (searches: Array<Alert | RegExp | string>, revert = false) =>
    (alert: Alert): boolean => {
        for (const search of searches) {
            if (typeof search === "string") {
                if (search === alert.message_id) return !revert;
            } else if (search instanceof Alert) {
                if (
                    search.message_id === alert.message_id &&
                    deepEqual(search.params, alert.params)
                )
                    return !revert;
            } else if (search.test(alert.message_id)) {
                return !revert;
            }
        }
        return revert;
    };

const create_alert_store = (type: AlertType): AlertStore => {
    const { set, subscribe, update } = writable<Alert[]>([]);

    return {
        add_from_api: (...alerts: ApiAlert[]): void => {
            update((current) => {
                const new_alerts = alerts
                    .map(Alert.from_api_alert(type))
                    .filter(alert_filter(current, true));
                return [...new_alerts, ...current];
            });
        },
        clear: (...searches: Array<Alert | RegExp | string>): void => {
            if (searches.length > 0) {
                update((current) =>
                    current.filter(alert_filter(searches, true)),
                );
            } else {
                set([]);
            }
        },
        set,
        subscribe,
        update,
    };
};

export const errors = create_alert_store("error");
export const warnings = create_alert_store("warning");

export const got_errors: Readable<
    (...searches: Array<RegExp | string>) => boolean
> = derived(
    errors,
    ($errors) =>
        (...searches: Array<RegExp | string>) =>
            $errors.some(alert_filter(searches)),
);
