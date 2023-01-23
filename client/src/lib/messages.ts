import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

const config = { default_message_timeout: 10_000 };
export interface InfoMessage {
    message: string;
    symbol: symbol;
    timeout?: ReturnType<typeof setTimeout>;
}

interface InfoMessageStore extends Readable<InfoMessage[]> {
    clear: () => void;
    show: (...messages: string[]) => void;
    showTimed: (...messages: string[]) => void;
}

const create_info_message_store = (): InfoMessageStore => {
    const { set, subscribe, update } = writable<InfoMessage[]>([]);

    const add = (time: number | null, messages: string[]): void => {
        update((current) => {
            const newMessages: InfoMessage[] = [];

            for (const new_message of messages) {
                const existing = current.find(
                    ({ message }) => message === new_message,
                );
                if (existing) {
                    if (existing.timeout) clearTimeout(existing.timeout);
                    if (time) {
                        existing.timeout = setTimeout(() => {
                            update((old) =>
                                old.filter(
                                    (messageData) =>
                                        messageData.symbol !== existing.symbol,
                                ),
                            );
                        }, time);
                    }
                } else {
                    const symbol = Symbol("InfoMessage");
                    const new_info_message: InfoMessage = {
                        symbol,
                        message: new_message,
                    };
                    if (time) {
                        new_info_message.timeout = setTimeout(() => {
                            update((old) =>
                                old.filter(
                                    (message) => message.symbol !== symbol,
                                ),
                            );
                        }, time);
                    }
                    newMessages.push(new_info_message);
                }
            }
            return [...newMessages, ...current];
        });
    };

    return {
        clear: (): void => {
            set([]);
        },
        show: (...messages: string[]): void => {
            add(null, messages);
        },
        showTimed: (...messages: string[]): void => {
            add(config.default_message_timeout, messages);
        },
        subscribe,
    };
};

export const info_messages = create_info_message_store();
