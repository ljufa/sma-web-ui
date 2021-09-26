FROM nginx:alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY public /usr/share/nginx/html/
RUN mkdir /usr/share/nginx/html/pkg
COPY pkg /usr/share/nginx/html/pkg
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]