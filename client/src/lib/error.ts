export class DodatokError extends Error {
    public readonly source: string;
    public readonly id: string;

    public constructor(source: string, id: string) {
        super(`${source}.errors.${id}`);
        this.source = source;
        this.id = id;
    }
}
