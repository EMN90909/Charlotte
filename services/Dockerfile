FROM node:22-alpine
WORKDIR /app
COPY package.json ./
COPY src ./src
COPY updates ./updates
USER node
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=8087
EXPOSE 8087
CMD ["node","src/server.mjs"]
