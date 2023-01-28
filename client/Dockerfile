FROM node:19.4.0-alpine3.17
RUN npm install --global pnpm
WORKDIR /program
CMD pnpm install
CMD pnpm --recursive run build
ENTRYPOINT pnpm run dev --host
